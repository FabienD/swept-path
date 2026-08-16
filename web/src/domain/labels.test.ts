import { describe, expect, it } from "vitest";
import type { Preferences } from "../i18n/preferences";
import { length, moves, verdictDetail, verdictHeadline, verdictNuance } from "./labels";
import type { Verdict } from "./verdict";

const FR: Preferences = { locale: "fr", units: "metric" };
const US: Preferences = { locale: "en", units: "us" };
/** The pairing that proves the two settings are independent. */
const FR_INCHES: Preferences = { locale: "fr", units: "us" };

const passes = (patch: Partial<Extract<Verdict, { outcome: "passes" }>> = {}): Verdict => ({
  outcome: "passes",
  tone: "hairline",
  moves: 2,
  clearance: 0.045,
  confidence: "heuristic",
  tightestElsewhere: null,
  ...patch,
});

describe("writing a length", () => {
  it("uses the decimal mark of the language", () => {
    expect(length(0.045, "clearance", FR)).toBe("4,5 cm");
    expect(length(0.045, "clearance", US)).toBe("1.8 in");
  });

  it("lets language and units be chosen separately", () => {
    // A French driver measuring an imported car: inches, in French.
    expect(length(0.045, "clearance", FR_INCHES)).toBe("1,8 in");
  });

  it("gives a street feet and a vehicle inches", () => {
    expect(length(5.9, "distance", US)).toBe("19.4 ft");
    expect(length(2.58, "dimension", US)).toBe("101.6 in");
  });
});

describe("counting moves", () => {
  it("agrees on plurals in both languages", () => {
    expect(moves(1, "fr")).toBe("1 manœuvre");
    expect(moves(3, "fr")).toBe("3 manœuvres");
    expect(moves(1, "en")).toBe("1 move");
    expect(moves(3, "en")).toBe("3 moves");
  });
});

describe("the headline", () => {
  it("answers the question that was asked, in either language", () => {
    expect(verdictHeadline(passes(), "fr")).toBe("Ça passe.");
    expect(verdictHeadline(passes(), "en")).toBe("It fits.");
    expect(verdictHeadline({ outcome: "blocked" }, "fr")).toBe("Ça ne passe pas.");
    expect(verdictHeadline({ outcome: "blocked" }, "en")).toBe("It does not fit.");
  });

  it("does not claim impossibility when nothing was proved", () => {
    expect(verdictHeadline({ outcome: "unproven" }, "fr")).toBe("Rien trouvé.");
    expect(verdictHeadline({ outcome: "unproven" }, "en")).toBe("Nothing found.");
  });
});

describe("the nuance", () => {
  it("qualifies a tight pass without contradicting it", () => {
    expect(verdictNuance(passes({ tone: "hairline" }), "fr")).toBe("De justesse.");
    expect(verdictNuance(passes({ tone: "hairline" }), "en")).toBe("Barely.");
    expect(verdictNuance(passes({ tone: "roomy" }), "en")).toBe("Easily.");
  });

  it("stays silent when there is nothing to add", () => {
    expect(verdictNuance(passes({ tone: "fine" }), "fr")).toBeNull();
    expect(verdictNuance(passes({ tone: "fine" }), "en")).toBeNull();
  });

  it("says outright that an empty heuristic result proves nothing", () => {
    expect(verdictNuance({ outcome: "unproven" }, "fr")).toBe("Sans preuve.");
    expect(verdictNuance({ outcome: "unproven" }, "en")).toBe("Not proven.");
  });

  it("names width as the reason when width is the reason", () => {
    const narrow: Verdict = { outcome: "too-narrow", opening: 1.9, vehicleWidth: 2.029 };
    expect(verdictNuance(narrow, "fr")).toBe("Le véhicule est trop large.");
    expect(verdictNuance(narrow, "en")).toBe("The vehicle is too wide.");
  });
});

describe("the detail under the sentence", () => {
  it("never gives a margin without saying where it came from", () => {
    // The rule from CLAUDE.md: the tool reports centimetres, so the
    // confidence behind them is part of the result, not a footnote.
    expect(verdictDetail(passes(), FR)).toContain("4,5 cm");
    expect(verdictDetail(passes(), FR)).toContain("recherche heuristique");
    expect(verdictDetail(passes(), US)).toContain("1.8 in");
    expect(verdictDetail(passes(), US)).toContain("heuristic search");
  });

  it("counts the manoeuvres", () => {
    expect(verdictDetail(passes({ moves: 1 }), FR)).toContain("1 manœuvre");
    expect(verdictDetail(passes({ moves: 3 }), US)).toContain("3 moves");
  });

  it("says where a tighter point sits when it is not the gateway", () => {
    const detail = verdictDetail(passes({ tightestElsewhere: 0.01 }), FR);
    expect(detail).toContain("1,0 cm");
    expect(detail).toContain("pas dans le passage");
    expect(verdictDetail(passes({ tightestElsewhere: 0.01 }), US)).toContain(
      "not in the gateway",
    );
  });

  it("explains that an exhausted budget is not a proof", () => {
    expect(verdictDetail({ outcome: "unproven" }, FR)).toContain("ne prouve pas");
    expect(verdictDetail({ outcome: "unproven" }, US)).toContain("does not prove");
  });

  it("gives both widths when the vehicle is simply too wide", () => {
    const narrow: Verdict = { outcome: "too-narrow", opening: 1.9, vehicleWidth: 2.029 };
    expect(verdictDetail(narrow, FR)).toContain("2,03 m");
    expect(verdictDetail(narrow, FR)).toContain("1,90 m");
    // In inches, since a gateway is a dimension and not a street.
    expect(verdictDetail(narrow, US)).toContain("79.9 in");
    expect(verdictDetail(narrow, US)).toContain("74.8 in");
  });
});
