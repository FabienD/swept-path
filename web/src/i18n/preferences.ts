/**
 * Which language and which units, remembered between visits.
 *
 * Two settings rather than one, because the pairings that look odd are real:
 * a French driver measuring an imported car wants inches in French, and an
 * English speaker living in France wants metres in English. Choosing English
 * moves the units to imperial once, as a first guess; after that they are
 * independent.
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
 * Only the United States gets inches. Britain reads English and measures its
 * gateways in metres, so language alone is not enough to decide units — which
 * is the same reason the two settings stay separate.
 */
export function detectPreferences(language: string | undefined): Preferences {
  if (!language) return DEFAULT;
  const tag = language.toLowerCase();
  if (tag.startsWith("fr")) return { locale: "fr", units: "metric" };
  return { locale: "en", units: tag === "en-us" ? "us" : "metric" };
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
