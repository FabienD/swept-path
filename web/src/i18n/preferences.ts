/**
 * Which language and which units, remembered between visits.
 *
 * Two settings, fully independent. The pairings that look odd are the real
 * ones: a French driver measuring an imported car wants inches in French, and
 * an English speaker living in France wants metres in English.
 *
 * Language is guessed from the browser; units never are. Metres are the
 * default for everyone, because they are what the tool computes in and what
 * the gateway was almost certainly measured with — imperial is a choice
 * someone makes, not one made for them.
 */
import type { UnitSystem } from "../domain/units";

export type Locale = "fr" | "en";

export interface Preferences {
  locale: Locale;
  units: UnitSystem;
}

/** Where the choice is kept. Namespaced, since localStorage is shared. */
export const STORAGE_KEY = "swept-path.preferences";

/** The little of `localStorage` this module needs, so tests need no browser. */
export interface Storage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const DEFAULT: Preferences = { locale: "fr", units: "metric" };

/**
 * What to show someone who has never chosen.
 *
 * The language follows the browser; the units do not follow the language.
 * Even in the United States, a gateway is a thing someone goes out and
 * measures, and the tape they own is far more likely to be metric than the
 * page's language suggests.
 */
export function detectPreferences(language: string | undefined): Preferences {
  if (!language) return DEFAULT;
  return language.toLowerCase().startsWith("fr")
    ? { locale: "fr", units: "metric" }
    : { locale: "en", units: "metric" };
}

const isLocale = (value: unknown): value is Locale => value === "fr" || value === "en";
const isUnits = (value: unknown): value is UnitSystem =>
  value === "metric" || value === "us";

/** The choice made here before, or a guess from the browser. */
export function loadPreferences(
  storage: Storage,
  language: string | undefined,
): Preferences {
  try {
    const held = storage.getItem(STORAGE_KEY);
    if (held) {
      const parsed: unknown = JSON.parse(held);
      if (parsed && typeof parsed === "object") {
        const { locale, units } = parsed as Record<string, unknown>;
        if (isLocale(locale) && isUnits(units)) return { locale, units };
      }
    }
  } catch {
    // A half-written entry, a hand-edited one, or a browser refusing storage
    // altogether. None of them is a reason to fail before drawing anything.
  }
  return detectPreferences(language);
}

/** Keeps the choice for next time, silently giving up if that is refused. */
export function savePreferences(preferences: Preferences, storage: Storage): void {
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify(preferences));
  } catch {
    // Private browsing throws here. The interface still works; it just forgets.
  }
}
