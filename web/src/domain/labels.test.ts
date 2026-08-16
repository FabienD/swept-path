import { describe, expect, it } from "vitest";
import { verdictDetail, verdictHeadline, verdictNuance } from "./labels";
import type { Verdict } from "./verdict";

const passes = (patch: Partial<Extract<Verdict, { outcome: "passes" }>> = {}): Verdict => ({
  outcome: "passes",
  tone: "hairline",
  moves: 2,
  clearance: 0.045,
  confidence: "heuristic",
  tightestElsewhere: null,
  ...patch,
});

describe("the headline", () => {
  it("answers the question that was asked", () => {
    expect(verdictHeadline(passes())).toBe("Ça passe.");
    expect(verdictHeadline({ outcome: "blocked" })).toBe("Ça ne passe pas.");
  });

  it("does not claim impossibility when nothing was proved", () => {
    expect(verdictHeadline({ outcome: "unproven" })).toBe("Rien trouvé.");
  });
});

describe("the nuance", () => {
  it("qualifies a tight pass without contradicting it", () => {
    expect(verdictNuance(passes({ tone: "hairline" }))).toBe("De justesse.");
    expect(verdictNuance(passes({ tone: "snug" }))).toBe("Sans confort.");
    expect(verdictNuance(passes({ tone: "roomy" }))).toBe("Largement.");
  });

  it("stays silent when there is nothing to add", () => {
    expect(verdictNuance(passes({ tone: "fine" }))).toBeNull();
  });

  it("says outright that an empty heuristic result proves nothing", () => {
    expect(verdictNuance({ outcome: "unproven" })).toBe("Sans preuve.");
  });
});

describe("the detail under the sentence", () => {
  it("never gives a margin without saying where it came from", () => {
    // The rule from CLAUDE.md: the tool reports centimetres, so the
    // confidence behind them is part of the result, not a footnote.
    const detail = verdictDetail(passes());
    expect(detail).toContain("4,5 cm");
    expect(detail).toContain("recherche heuristique");
  });

  it("counts the manoeuvres", () => {
    expect(verdictDetail(passes({ moves: 1 }))).toContain("1 manœuvre");
    expect(verdictDetail(passes({ moves: 3 }))).toContain("3 manœuvres");
  });

  it("says where a tighter point sits when it is not the gateway", () => {
    const detail = verdictDetail(passes({ tightestElsewhere: 0.01 }));
    expect(detail).toContain("1,0 cm");
    expect(detail).toContain("pas dans le passage");
  });

  it("explains that an exhausted budget is not a proof", () => {
    expect(verdictDetail({ outcome: "unproven" })).toContain("ne prouve pas");
  });
});
