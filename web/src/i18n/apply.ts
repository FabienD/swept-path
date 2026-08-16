/**
 * Putting a page into one language.
 *
 * Shared by the three pages, which is the whole reason it is not in
 * `main.ts`: the simulator, the documentation and the disclaimer all carry
 * `data-i18n` markup and must all honour the same stored choice. Duplicating
 * this would mean a key that works on one page and renders blank on another.
 */
import type { Magnitude } from "../domain/units";
import { unitOf } from "../domain/units";
import type { TextKey } from "./dictionary";
import { text } from "./dictionary";
import type { Preferences } from "./preferences";

/** Labels, placeholders, ARIA labels and unit suffixes, in one language. */
export function applyLanguage(preferences: Preferences): void {
  const { locale, units } = preferences;

  for (const node of document.querySelectorAll<HTMLElement>("[data-i18n]")) {
    node.textContent = text(locale, node.dataset["i18n"] as TextKey);
  }
  for (const node of document.querySelectorAll<HTMLElement>("[data-i18n-placeholder]")) {
    node.setAttribute(
      "placeholder",
      text(locale, node.dataset["i18nPlaceholder"] as TextKey),
    );
  }
  for (const node of document.querySelectorAll<HTMLElement>("[data-i18n-aria]")) {
    node.setAttribute("aria-label", text(locale, node.dataset["i18nAria"] as TextKey));
  }
  for (const node of document.querySelectorAll<HTMLElement>("[data-unit]")) {
    node.textContent = unitOf(node.dataset["unit"] as Magnitude, units);
  }

  document.documentElement.lang = locale;
}
