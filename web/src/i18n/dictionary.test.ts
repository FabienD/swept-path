import { describe, expect, it } from "vitest";
import { TEXT_KEYS, text } from "./dictionary";

describe("the translations", () => {
  it("covers every key in both languages", () => {
    // A missing key would render as blank or as the key itself, and only on
    // the page nobody happened to open in that language.
    for (const key of TEXT_KEYS) {
      expect(text("fr", key), `fr/${key}`).toBeTruthy();
      expect(text("en", key), `en/${key}`).toBeTruthy();
    }
  });

  it("says something different in each language", () => {
    // Not a style rule: an entry identical in both is almost always one that
    // was copied across and never translated. The exceptions are named.
    const shared = new Set(["play.pause"]);
    const untranslated = TEXT_KEYS.filter(
      (key) => !shared.has(key) && text("fr", key) === text("en", key),
    );
    expect(untranslated).toEqual([]);
  });

  it("carries no unit in a field label", () => {
    // Units are appended from the current system, so a label reading
    // "Passage m" would show "Passage m" beside a figure in inches.
    for (const key of TEXT_KEYS) {
      for (const locale of ["fr", "en"] as const) {
        expect(text(locale, key), `${locale}/${key}`).not.toMatch(
          /\b(m|cm|in|ft)$/,
        );
      }
    }
  });
});
