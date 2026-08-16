import { describe, expect, it } from "vitest";
import { STORAGE_KEY, detectPreferences, loadPreferences, savePreferences } from "./preferences";

/** A storage that behaves, and one that does not. */
const fake = (initial: Record<string, string> = {}) => {
  const held = { ...initial };
  return {
    getItem: (k: string) => held[k] ?? null,
    setItem: (k: string, v: string) => {
      held[k] = v;
    },
    held,
  };
};

describe("guessing from the browser", () => {
  it("gives French to a French browser", () => {
    expect(detectPreferences("fr-FR")).toEqual({ locale: "fr", units: "metric" });
    expect(detectPreferences("fr-CA")).toEqual({ locale: "fr", units: "metric" });
  });

  it("never guesses imperial, not even in the United States", () => {
    // Metres are what the tool computes in and, far more often than the
    // page's language suggests, what the gateway was measured with. Imperial
    // is a choice someone makes, never one made for them.
    expect(detectPreferences("en-US")).toEqual({ locale: "en", units: "metric" });
    expect(detectPreferences("en-GB")).toEqual({ locale: "en", units: "metric" });
  });

  it("falls back to English and metres for a language it does not speak", () => {
    expect(detectPreferences("de-DE")).toEqual({ locale: "en", units: "metric" });
  });

  it("falls back to French when the browser says nothing", () => {
    // The project is French, so an unknown visitor gets its own language.
    expect(detectPreferences(undefined)).toEqual({ locale: "fr", units: "metric" });
    expect(detectPreferences("")).toEqual({ locale: "fr", units: "metric" });
  });
});

describe("remembering a choice", () => {
  it("prefers what was chosen over what was guessed", () => {
    const store = fake({ [STORAGE_KEY]: '{"locale":"fr","units":"us"}' });
    expect(loadPreferences(store, "fr-FR")).toEqual({ locale: "fr", units: "us" });
  });

  it("guesses when nothing was ever chosen", () => {
    expect(loadPreferences(fake(), "en-US")).toEqual({ locale: "en", units: "metric" });
  });

  it("guesses rather than throw on a stored value it cannot read", () => {
    // A half-written or hand-edited entry must not take the interface down
    // before it has drawn anything.
    expect(loadPreferences(fake({ [STORAGE_KEY]: "{" }), "fr-FR")).toEqual({
      locale: "fr",
      units: "metric",
    });
    expect(
      loadPreferences(fake({ [STORAGE_KEY]: '{"locale":"martian"}' }), "fr-FR"),
    ).toEqual({ locale: "fr", units: "metric" });
  });

  it("survives a browser that refuses storage", () => {
    // Private browsing throws on setItem rather than returning anything.
    const hostile = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
    };
    expect(() => savePreferences({ locale: "en", units: "us" }, hostile)).not.toThrow();
    expect(loadPreferences(hostile, "fr-FR")).toEqual({ locale: "fr", units: "metric" });
  });

  it("writes back what can be read again", () => {
    const store = fake();
    savePreferences({ locale: "en", units: "us" }, store);
    expect(loadPreferences(store, "fr-FR")).toEqual({ locale: "en", units: "us" });
  });
});
