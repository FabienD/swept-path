import { describe, expect, it } from "vitest";
import type { ManeuverDto, SolveResponse } from "./types";
import {
  clearanceCeiling,
  gaugeFraction,
  refuseOnWidth,
  verdictOf,
} from "./verdict";

function maneuver(patch: Partial<ManeuverDto> = {}): ManeuverDto {
  return {
    poses: [],
    min_clearance: 0.045,
    min_clearance_in_gateway: 0.045,
    metres_overhanging: 0,
    metres_under_25cm: 1,
    metres_under_10cm: 1,
    distance: 11.4,
    moves: 2,
    confidence: "heuristic",
    ...patch,
  };
}

const answered = (alternatives: ManeuverDto[]): SolveResponse => ({
  alternatives,
  budget_exhausted: false,
});

describe("the geometric ceiling", () => {
  it("is half of what the opening leaves beside the vehicle", () => {
    // The reference gateway: 2,29 m of opening, 2,029 m across the mirrors.
    // No trajectory does better than 13,05 cm, whatever the manoeuvre count.
    expect(clearanceCeiling(2.29, 2.029)).toBeCloseTo(0.1305, 6);
  });

  it("is nothing at all when the vehicle is wider than the opening", () => {
    // A negative ceiling would put the gauge's zero above its maximum and
    // draw the marker off the far end of the scale.
    expect(clearanceCeiling(1.9, 2.029)).toBe(0);
  });
});

describe("where the marker sits on the gauge", () => {
  it("places a clearance against its ceiling", () => {
    expect(gaugeFraction(0.045, 0.1305)).toBeCloseTo(0.3448, 3);
  });

  it("never leaves the scale", () => {
    expect(gaugeFraction(0.5, 0.1305)).toBe(1);
    expect(gaugeFraction(-0.01, 0.1305)).toBe(0);
  });

  it("reads as empty when there is no room to be had", () => {
    // Dividing by a zero ceiling would yield NaN or Infinity, and the marker
    // would vanish rather than sit at the left edge.
    expect(gaugeFraction(0, 0)).toBe(0);
  });
});

describe("the verdict on a search that found something", () => {
  it("calls a hairline margin what it is", () => {
    const v = verdictOf(answered([maneuver({ min_clearance_in_gateway: 0.045 })]));
    expect(v.outcome).toBe("passes");
    expect(v.outcome === "passes" && v.tone).toBe("hairline");
  });

  it("separates snug from hairline at ten centimetres", () => {
    const snug = verdictOf(answered([maneuver({ min_clearance_in_gateway: 0.10 })]));
    expect(snug.outcome === "passes" && snug.tone).toBe("snug");
  });

  it("calls half a metre roomy", () => {
    const v = verdictOf(answered([maneuver({ min_clearance_in_gateway: 0.6 })]));
    expect(v.outcome === "passes" && v.tone).toBe("roomy");
  });

  it("judges on the gateway, not on the tightest point anywhere", () => {
    // The driver asked about the gateway. A path that grazes a parked car on
    // the way in is a different warning, and it is carried separately.
    const v = verdictOf(
      answered([maneuver({ min_clearance: 0.01, min_clearance_in_gateway: 0.6 })]),
    );
    expect(v.outcome === "passes" && v.tone).toBe("roomy");
    expect(v.outcome === "passes" && v.tightestElsewhere).toBeCloseTo(0.01, 6);
  });

  it("carries no elsewhere when the gateway is the tightest point", () => {
    const v = verdictOf(answered([maneuver({ min_clearance: 0.045 })]));
    expect(v.outcome === "passes" && v.tightestElsewhere).toBeNull();
  });

  it("reports the first alternative, which is the roomiest", () => {
    const v = verdictOf(answered([maneuver({ moves: 2 }), maneuver({ moves: 1 })]));
    expect(v.outcome === "passes" && v.moves).toBe(2);
  });
});

describe("the verdict on a search that found nothing", () => {
  it("calls it impossible only when the search was exhaustive", () => {
    expect(verdictOf({ alternatives: [], budget_exhausted: false }).outcome).toBe(
      "blocked",
    );
  });

  it("refuses to prove absence from a budget that ran out", () => {
    // The heuristic planner missing an entry says nothing about whether one
    // exists. Stating otherwise would be the interface inventing certainty.
    expect(verdictOf({ alternatives: [], budget_exhausted: true }).outcome).toBe(
      "unproven",
    );
  });
});

describe("refusing before any search", () => {
  it("refuses a vehicle wider than the opening", () => {
    // The reference gateway narrowed below the LBX across its mirrors.
    const refusal = refuseOnWidth(1.9, 2.029);
    expect(refusal?.outcome).toBe("too-narrow");
  });

  it("refuses a vehicle that fits with nothing at all to spare", () => {
    // Equality is not fitting: it is touching both posts at once, for the
    // whole depth of the passage.
    expect(refuseOnWidth(2.029, 2.029)?.outcome).toBe("too-narrow");
  });

  it("lets a vehicle that fits through be searched for", () => {
    expect(refuseOnWidth(2.29, 2.029)).toBeNull();
  });

  it("judges on the width the run would actually use", () => {
    // Folding the mirrors is exactly what gets a car through a gate it does
    // not otherwise fit, so the refusal must not be made on the wider figure
    // when the run is set to folded.
    const opening = 1.95;
    expect(refuseOnWidth(opening, 2.029)?.outcome).toBe("too-narrow");
    expect(refuseOnWidth(opening, 1.865)).toBeNull();
  });

  it("carries both widths, so the refusal can say why", () => {
    const refusal = refuseOnWidth(1.9, 2.029);
    expect(refusal).toEqual({
      outcome: "too-narrow",
      opening: 1.9,
      vehicleWidth: 2.029,
    });
  });

  it("agrees with the ceiling it is derived from", () => {
    // The refusal is exactly the case where the ceiling has nothing left to
    // give. Two ways of saying the same geometry; they must not drift.
    for (const [opening, width] of [
      [1.9, 2.029],
      [2.029, 2.029],
      [2.29, 2.029],
      [3.0, 2.029],
    ] as const) {
      const refused = refuseOnWidth(opening, width) !== null;
      expect(refused).toBe(clearanceCeiling(opening, width) <= 0);
    }
  });
});
