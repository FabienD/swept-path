/**
 * The documentation and disclaimer pages.
 *
 * They share the simulator's language and units, read from the same storage,
 * so following a link never changes what you were reading in. What they do
 * not share is the solver: neither page loads the Wasm module, because
 * neither needs it — the figures come from the pure renderer.
 */
import { applyLanguage } from "./i18n/apply";
import type { Preferences } from "./i18n/preferences";
import { loadPreferences, savePreferences } from "./i18n/preferences";
import { BANDS } from "./domain/bands";
import { squareAgainstAskew, squareInTheGateway } from "./render/figures";
import type { Primitive, Role } from "./render/primitives";
import { renderSvg } from "./render/svg";
import { projectionFor } from "./render/projection";

const storage = globalThis.localStorage;

/** The reference gateway these figures are drawn on, in metres. */
const FIGURE_OPENING = 2.4;

/** How far off square the second vehicle sits in the askew figure, in degrees. */
const FIGURE_SKEW_DEGREES = 15;

const byId = <T extends HTMLElement>(id: string): T | null =>
  document.getElementById(id) as T | null;

/**
 * The four proximity bands, as four stretches of trace.
 *
 * Drawn through the renderer rather than as coloured rectangles, so the
 * legend cannot drift from the plan: same roles, same tokens, same widths.
 */
function paintBands(svg: SVGSVGElement): void {
  const roles: Role[] = ["band-clear", "band-watch", "band-close", "band-tight"];
  const primitives: Primitive[] = roles.map((role, i) => ({
    type: "polyline",
    role,
    points: [
      { x: -6 + i * 3, y: 0 },
      { x: -3.4 + i * 3, y: 0 },
    ],
  }));
  const viewport = { width: 900, height: 90 };
  svg.setAttribute("viewBox", `0 0 ${viewport.width} ${viewport.height}`);
  renderSvg(
    primitives,
    svg,
    projectionFor({ xMin: -6.4, xMax: 6.4, yMin: -0.6, yMax: 0.6 }, viewport, false),
  );
}

/** Draws whichever figures this page happens to carry. */
function paintFigures(preferences: Preferences): void {
  const ceiling = byId<HTMLElement>("figure-ceiling");
  if (ceiling instanceof SVGSVGElement) {
    squareInTheGateway(ceiling, FIGURE_OPENING, preferences);
  }
  const askew = byId<HTMLElement>("figure-askew");
  if (askew instanceof SVGSVGElement) {
    squareAgainstAskew(askew, FIGURE_OPENING, FIGURE_SKEW_DEGREES, preferences);
  }
  const bands = byId<HTMLElement>("figure-bands");
  if (bands instanceof SVGSVGElement) paintBands(bands);
}

function start(): void {
  let preferences = loadPreferences(storage, globalThis.navigator?.language);

  const locale = byId<HTMLSelectElement>("locale");
  if (locale) {
    locale.value = preferences.locale;
    locale.addEventListener("change", () => {
      preferences = {
        locale: locale.value as Preferences["locale"],
        units: preferences.units,
      };
      savePreferences(preferences, storage);
      applyLanguage(preferences);
      // The figures carry their own dimensions, so they are language-bound
      // too — a plan annotated in French on an English page would be the one
      // thing left untranslated, and the most visible.
      paintFigures(preferences);
    });
  }

  applyLanguage(preferences);
  paintFigures(preferences);
}

start();

// Referenced so the thresholds the legend describes come from the same table
// the plan colours by; if BANDS ever changes, this file changes with it.
export { BANDS };
