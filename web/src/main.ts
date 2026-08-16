import { BANDS } from "./domain/bands";
import {
  confidenceLabel,
  errorMessage,
  leafTooOpen,
  length,
  minRoadResult,
  missingMeasurements,
  moves,
  searchProgress,
  underThreshold,
  verdictDetail,
  verdictHeadline,
  verdictNuance,
} from "./domain/labels";
import type { ErrorDto, ManeuverDto, SceneDto, VehicleDto } from "./domain/types";
import type { VehiclePreset } from "./domain/vehicles";
import { VEHICLES, searchVehicles, vehicleById } from "./domain/vehicles";
import { NONE, commit, nextHighlight } from "./ui/combobox";
import type { Verdict } from "./domain/verdict";
import {
  clearanceCeiling,
  gaugeFraction,
  refuseOnWidth,
  verdictOf,
} from "./domain/verdict";
import { pathToPrimitives } from "./render/path";
import {
  elapsedFor,
  poseAt,
  positionAt,
  timelineOf,
  totalDuration,
} from "./render/playback";
import { projectionFor } from "./render/projection";
import { boundsFor, sceneToPrimitives } from "./render/scene";
import { renderSvg } from "./render/svg";
import { createStore } from "./state/store";
import { arrivesFromTheRight, readRequest, readScene } from "./ui/form";
import { CANCELLED, SolverClient } from "./worker/client";
import { text } from "./i18n/dictionary";
import type { TextKey } from "./i18n/dictionary";
import type { Preferences } from "./i18n/preferences";
import { loadPreferences, savePreferences } from "./i18n/preferences";
import type { Magnitude, UnitSystem } from "./domain/units";
import { fromDisplay, stepFor, toDisplay, unitOf } from "./domain/units";

const VIEWPORT = { width: 1000, height: 600 };
const client = new SolverClient();
// Instant queries get their own worker. They must neither cancel a search in
// flight nor be cancelled by one — which is exactly what happened when
// clearResult() started cancelling: it killed the angle query fired a line
// earlier, and the slider was never bounded.
const probe = new SolverClient();

const preferencesStorage: Pick<globalThis.Storage, "getItem" | "setItem"> =
  globalThis.localStorage;

const store = createStore({
  /** Language and units, restored from the last visit or guessed. */
  preferences: loadPreferences(
    preferencesStorage,
    globalThis.navigator?.language,
  ) as Preferences,
  busy: false,
  /**
   * The judged result, or null when there is nothing to judge.
   *
   * Kept apart from `message`: one is an answer to the question asked, the
   * other is the interface talking about itself — "calcul en cours", a
   * rejected measurement, the minimum carriageway. Showing them in the same
   * slot is what made every state look equally important.
   */
  outcome: null as Verdict | null,
  message: "",
  progress: "",
  alternatives: [] as ManeuverDto[],
  selected: 0,
  position: 1,
  playing: false,
  maxAngleDegrees: null as number | null,
});

const byId = <T extends HTMLElement>(id: string): T | null =>
  document.getElementById(id) as T | null;

/* ------------------------------------------------------------------ i18n */

/**
 * Every numeric field, with what it measures.
 *
 * Read from the markup rather than listed here, so a field added to the page
 * cannot be forgotten by this file — it would show a figure with no unit and
 * stop converting, silently, which is the worst way to be wrong about a
 * measurement.
 */
function measuredFields(): { input: HTMLInputElement; magnitude: Magnitude }[] {
  return [...document.querySelectorAll<HTMLInputElement>("input[data-magnitude]")].map(
    (input) => ({ input, magnitude: input.dataset["magnitude"] as Magnitude }),
  );
}

/** Puts the page into one language: labels, placeholders, and the units. */
function applyLanguage(preferences: Preferences): void {
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

/**
 * Rewrites every measurement into the other system.
 *
 * The fields hold what the reader typed, in the unit they typed it in, so the
 * switch has to convert them — leaving 2.29 in a field now labelled "in"
 * would silently turn a 2,29 m gateway into a 5,8 cm one. Going through
 * metres both ways means the stored measurement never changes; only how it
 * is written does.
 */
function convertFields(from: UnitSystem, to: UnitSystem): void {
  if (from === to) return;
  for (const { input, magnitude } of measuredFields()) {
    input.step = String(stepFor(magnitude, to));
    if (input.value === "") continue;
    const held = fromDisplay(input.valueAsNumber, magnitude, from);
    if (Number.isNaN(held)) continue;
    const shown = toDisplay(held, magnitude, to);
    // Rounded to the step, so the field shows a figure someone could have
    // typed rather than the full float of a conversion.
    const step = stepFor(magnitude, to);
    input.value = String(Math.round(shown / step) * step);
  }
}

/* -------------------------------------------------------------- playback */

/**
 * The clock. Everything it does is push `position` into the store; the plan
 * redraws from that, exactly as it does when the scrubber is dragged.
 *
 * The arithmetic — how long a trip takes, where the pauses fall, which pose
 * sits at a position — belongs to `render/playback.ts`, which is pure and
 * tested. This is only the part that needs a browser.
 */
let frame = 0;

/** True when the visitor has asked their system for less movement. */
const stillnessWanted = (): boolean =>
  globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

/** The path being shown, measured, or null when there is none. */
function currentTimeline() {
  const { alternatives, selected } = store.get();
  const current = alternatives[selected];
  return current ? timelineOf(current.poses) : null;
}

function stopPlaying(): void {
  if (frame) cancelAnimationFrame(frame);
  frame = 0;
  store.set({ playing: false });
}

function startPlaying(): void {
  const timeline = currentTimeline();
  if (!timeline || timeline.length === 0) return;

  // Asked for stillness: show the finished épure rather than refuse. The
  // figure is the point; the animation is only how it gets drawn.
  if (stillnessWanted()) {
    store.set({ position: 1 });
    return;
  }

  // A finished playback replays from the start; a paused one carries on from
  // where it stopped, so pausing to look at something does not cost your place.
  const from = store.get().position >= 1 ? 0 : store.get().position;
  const total = totalDuration(timeline);
  const offset = elapsedFor(timeline, from);
  const began = performance.now() - offset;

  store.set({ playing: true, position: from });
  const tick = (now: number): void => {
    const elapsed = now - began;
    store.set({ position: positionAt(timeline, elapsed / 1000) });
    if (elapsed / 1000 >= total) {
      stopPlaying();
      return;
    }
    frame = requestAnimationFrame(tick);
  };
  frame = requestAnimationFrame(tick);
}

/* ------------------------------------------------------------------ draw */

/** Redraws the plan. Cheap enough to run on every change. */
function draw(): void {
  const svg = document.getElementById("plan");
  if (!(svg instanceof SVGSVGElement)) return;

  let scene: SceneDto;
  let vehicle: VehicleDto;
  try {
    scene = readScene(store.get().preferences.units);
    vehicle = readRequest(store.get().preferences.units).vehicle;
  } catch {
    return;
  }

  const projection = projectionFor(boundsFor(scene), VIEWPORT, arrivesFromTheRight());
  const primitives = [...sceneToPrimitives(scene, store.get().preferences)];

  const { alternatives, selected, position } = store.get();
  const current = alternatives[selected];
  if (current) {
    primitives.push(...pathToPrimitives(current, vehicle, position));
  }
  renderSvg(primitives, svg, projection);
}

/* --------------------------------------------------------------- reports */

const BAND_KEYS = [
  "band.clear",
  "band.watch",
  "band.close",
  "band.tight",
] as const satisfies readonly TextKey[];
const BAND_TOKENS = [
  "--color-band-clear",
  "--color-band-watch",
  "--color-band-close",
  "--color-band-tight",
] as const;

function renderLegend(show: boolean): void {
  const legend = byId("legend");
  if (!legend) return;
  if (!show) {
    legend.replaceChildren();
    return;
  }
  const preferences = store.get().preferences;
  const english = preferences.locale === "en";
  const bound = (metres: number) => length(metres, "clearance", preferences);

  legend.innerHTML = `${BAND_KEYS.map((key, i) => {
    const range =
      i === 0
        ? english
          ? `over ${bound(BANDS[0])}`
          : `plus de ${bound(BANDS[0])}`
        : i === 3
          ? english
            ? `under ${bound(BANDS[2])}`
            : `moins de ${bound(BANDS[2])}`
          : english
            ? `${bound(BANDS[i]!)} to ${bound(BANDS[i - 1]!)}`
            : `${bound(BANDS[i]!)} à ${bound(BANDS[i - 1]!)}`;
    return `<span class="flex items-center gap-1"><i class="inline-block h-2 w-4 rounded" style="background:var(${BAND_TOKENS[i]})"></i>${text(
      preferences.locale,
      key,
    )} (${range})</span>`;
  }).join("")}<span class="flex items-center gap-1"><i class="inline-block h-2 w-4 rounded" style="background:var(--color-overhang)"></i>${text(
    preferences.locale,
    "legend.overhang",
  )}</span><span class="ml-auto">${text(preferences.locale, "legend.gears")}</span>`;
}

function renderStats(maneuver: ManeuverDto): void {
  const stats = byId("stats");
  if (!stats) return;
  const card = (key: string, value: string) =>
    `<div class="rounded border border-line bg-panel px-3 py-2">
       <dt class="text-xs uppercase tracking-wide text-dim">${key}</dt>
       <dd class="mt-0.5 text-lg tabular-nums">${value}</dd>
     </div>`;

  const preferences = store.get().preferences;
  const room = (m: number) => length(m, "clearance", preferences);
  const far = (m: number) => length(m, "distance", preferences);
  const say = (key: TextKey) => text(preferences.locale, key);

  // Two clearances, because they answer different questions: the gateway is
  // what the driver asked about, the overall figure may be a curb metres away.
  stats.innerHTML = [
    card(say("stats.moves"), String(maneuver.moves)),
    card(say("stats.gatewayClearance"), room(maneuver.min_clearance_in_gateway)),
    card(say("stats.tripClearance"), room(maneuver.min_clearance)),
    card(say("stats.distance"), far(maneuver.distance)),
    // The thresholds are shown in the reader's unit too, so the cards and the
    // legend cannot disagree about where "close" begins.
    card(underThreshold(BANDS[1], preferences), far(maneuver.metres_under_25cm)),
    card(underThreshold(BANDS[2], preferences), far(maneuver.metres_under_10cm)),
    // Shown only when it has something to say: a zero on every ordinary
    // trajectory would teach nobody anything.
    ...(maneuver.metres_overhanging > 0
      ? [card(say("stats.overhang"), far(maneuver.metres_overhanging))]
      : []),
  ].join("");
}

/**
 * Clearance below which a computed margin is not worth trusting, in metres.
 *
 * ARBITRARY. The tinted share of the gauge, and the order of magnitude below
 * which the figure no longer survives how accurately anyone measures their
 * own gateway with a tape.
 */
const UNTRUSTWORTHY_M = 0.015;

/**
 * Places the margin on a scale that ends at the most the geometry allows.
 *
 * `(W − w) / 2` is the ceiling whatever the trajectory — the project's main
 * conclusion. Without it, "4,5 cm" is a figure with no scale: the reader
 * cannot tell whether it is nearly all that was available or a third of it.
 */
function renderGauge(verdict: Verdict | null): void {
  const gauge = byId("gauge");
  if (!gauge) return;

  if (!verdict || verdict.outcome !== "passes") {
    gauge.classList.add("hidden");
    return;
  }

  // Read from the form, which `clearResult` keeps in step with the result:
  // any edit wipes the verdict, so the widths shown are the widths solved.
  let ceiling: number;
  try {
    const scene = readScene(store.get().preferences.units);
    const opening = scene.right_post.inner_edge_x - scene.left_post.inner_edge_x;
    ceiling = clearanceCeiling(opening, readRequest(store.get().preferences.units).vehicle.mirror_width);
  } catch {
    gauge.classList.add("hidden");
    return;
  }

  gauge.classList.remove("hidden");
  const preferences = store.get().preferences;
  const value = byId("gauge-value");
  if (value) value.textContent = length(verdict.clearance, "clearance", preferences);
  const marker = byId("gauge-marker");
  if (marker) {
    marker.style.left = `${gaugeFraction(verdict.clearance, ceiling) * 100}%`;
  }
  const top = byId("gauge-ceiling");
  if (top) {
    top.textContent = `${length(ceiling, "clearance", preferences)} — ${text(
      preferences.locale,
      "gauge.ceiling",
    )}`;
  }
  // The tinted share is where a margin is thin in absolute terms, so it
  // shrinks as the ceiling grows rather than staying a fixed fraction.
  const track = gauge.querySelector<HTMLElement>(".gauge-track");
  if (track) {
    const share = ceiling > 0 ? Math.min(UNTRUSTWORTHY_M / ceiling, 1) : 1;
    track.style.setProperty("--gauge-danger", `${share * 100}%`);
  }
}

function renderAlternatives(): void {
  const box = byId("alternatives");
  if (!box) return;
  const { alternatives, selected, preferences } = store.get();
  if (alternatives.length === 0) {
    box.replaceChildren();
    return;
  }

  box.innerHTML = alternatives
    .map(
      (a, i) =>
        `<button type="button" data-index="${i}"
           class="rounded border px-3 py-2 text-left ${
             i === selected
               ? "border-accent bg-accent text-ink"
               : "border-line bg-panel text-fg"
           }">
           <span class="block text-sm font-medium">${moves(
             a.moves,
             preferences.locale,
           )}</span>
           <span class="block text-xs opacity-80">${length(
             a.min_clearance_in_gateway,
             "clearance",
             preferences,
           )} · ${confidenceLabel(a.confidence, preferences.locale)}</span>
         </button>`,
    )
    .join("");

  for (const button of box.querySelectorAll("button")) {
    button.addEventListener("click", () => {
      store.set({ selected: Number(button.dataset["index"]), position: 1 });
      const scrub = byId<HTMLInputElement>("scrub");
      if (scrub) scrub.value = "100";
    });
  }
}

/**
 * What the last rebuild of the choice-dependent parts was for.
 *
 * Measured, in case this looks like premature caution: recomputing the
 * timeline itself costs 15 µs a frame — 0.09 % of one — and is left alone.
 * It is the DOM rebuilding that had to stop, not the arithmetic.
 */
let shownAlternatives: readonly ManeuverDto[] | null = null;
let shownSelected = -1;

store.subscribe(() => {
  const { outcome, message, alternatives, selected, busy, progress, playing, preferences } =
    store.get();

  // The headline answers the question; the nuance qualifies it without
  // contradicting it. A free-form message has no headline — it is not an
  // answer — and takes the detail line on its own.
  const headline = byId("verdict-headline");
  if (headline) {
    headline.textContent = outcome ? verdictHeadline(outcome, preferences.locale) : "";
  }
  const nuance = byId("verdict-nuance");
  if (nuance) {
    nuance.textContent = outcome
      ? (verdictNuance(outcome, preferences.locale) ?? "")
      : "";
  }
  const detail = byId("verdict-detail");
  if (detail) {
    detail.textContent = outcome ? verdictDetail(outcome, preferences) : message;
    // A message is the interface speaking, and often a refusal: it would go
    // unread in the muted grey the detail line uses.
    detail.classList.toggle("text-dim", Boolean(outcome));
    detail.classList.toggle("text-fg", !outcome && message !== "");
  }
  renderGauge(outcome);

  const bar = byId("progress");
  if (bar) bar.classList.toggle("hidden", !busy);
  const note = byId("progress-note");
  if (note) note.textContent = progress;

  const run = byId("run");
  if (run) {
    run.textContent = text(preferences.locale, busy ? "action.stop" : "action.compute");
  }

  const current = alternatives[selected];

  // Which manoeuvre is on screen changes rarely; where the vehicle sits along
  // it changes sixty times a second. Rebuilding the alternatives on every
  // frame would re-create their nodes and re-attach their listeners under the
  // pointer — a click landing on a button that has just been replaced does
  // nothing at all. So the parts that depend only on the choice are rebuilt
  // only when the choice changes.
  if (alternatives !== shownAlternatives || selected !== shownSelected) {
    shownAlternatives = alternatives;
    shownSelected = selected;
    renderAlternatives();
    renderLegend(Boolean(current));
    if (current) renderStats(current);
    else byId("stats")?.replaceChildren();
  }

  const scrub = byId<HTMLInputElement>("scrub");
  if (scrub) {
    scrub.disabled = !current;
    // The clock owns the scrubber while it runs, so the handle tracks the
    // vehicle instead of sitting where it was last dropped.
    if (playing) scrub.value = String(Math.round(store.get().position * 100));
  }
  renderPlayback();

  draw();
});

/* ------------------------------------------------------------------ form */

function clearResult(): void {
  // Whatever is running answers a question that has just changed, so it is
  // abandoned rather than left to overwrite the screen with a stale verdict.
  client.cancel();
  // Including the playback: it would go on animating a path that no longer
  // matches the measurements on screen.
  stopPlaying();
  store.set({
    alternatives: [],
    selected: 0,
    position: 1,
    outcome: null,
    message: "",
    progress: "",
    busy: false,
  });
}

/**
 * Every numeric input a simulation needs filled.
 *
 * `mirror-width-folded` is absent on purpose: it only matters when the
 * mirrors are set to folded, and demanding it otherwise would flag a field
 * the run does not read.
 */
const REQUIRED_INPUTS: readonly string[] = [
  "opening",
  "post-depth",
  "post-width",
  "wall",
  "sidewalk",
  "curb",
  "road",
  "curb-height",
  "radius",
  "wheelbase",
  "length",
  "front-overhang",
  "body-width",
  "ground-clearance",
  "mirror-width",
];

/** Maps a field name the boundary rejects to the input that holds it. */
const FIELD_INPUTS: Record<string, string> = {
  wheelbase: "wheelbase",
  length: "length",
  front_overhang: "front-overhang",
  width: "body-width",
  mirror_width: "mirror-width",
  ground_clearance: "ground-clearance",
  min_turning_radius: "radius",
};

/**
 * The playback bar: the button, which gear, and the clearance right here.
 *
 * The live clearance is what ties the animation to the verdict — the figure
 * shown at the top is the smallest this one ever gets, and watching it fall
 * to that value is what makes it mean something.
 */
function renderPlayback(): void {
  const { alternatives, selected, position, playing, preferences } = store.get();
  const current = alternatives[selected];
  const say = (key: TextKey) => text(preferences.locale, key);

  const button = byId<HTMLButtonElement>("play");
  if (button) button.disabled = !current;
  const glyph = byId("play-glyph");
  const label = byId("play-label");
  if (glyph) glyph.textContent = playing ? "❚❚" : "▶";
  if (label) {
    label.textContent = say(
      playing ? "play.pause" : stillnessWanted() ? "play.still" : "play.play",
    );
  }

  const percent = byId("scrub-label");
  if (percent) percent.textContent = `${Math.round(position * 100)} %`;

  const gear = byId("gear");
  const live = byId("live-clearance");
  if (!current || current.poses.length === 0) {
    gear?.classList.add("hidden");
    if (live) live.textContent = "";
    return;
  }

  const pose = current.poses[poseAt(timelineOf(current.poses), position)]!;
  if (gear) {
    gear.classList.remove("hidden");
    gear.textContent = say(pose.reverse ? "gear.reverse" : "gear.forward");
    // Reverse wears white, from the vehicle's own reversing lamps; forward
    // wears the accent. The trace keeps the proximity colours, so the gear is
    // read on the vehicle and here, never on the path.
    gear.classList.toggle("bg-fg", pose.reverse);
    gear.classList.toggle("bg-accent", !pose.reverse);
    gear.classList.add("text-ink");
  }
  if (live) live.textContent = length(pose.clearance, "clearance", preferences);
}

/** Marks or clears one input as needing attention. */
function flag(id: string, missing: boolean): void {
  const input = byId<HTMLInputElement>(id);
  if (!input) return;
  input.classList.toggle("border-band-tight", missing);
  input.classList.toggle("bg-band-tight/10", missing);
  input.setAttribute("aria-invalid", missing ? "true" : "false");
  // A field flagged inside a closed disclosure is a message pointing at
  // something nobody can see. Opening it is the whole reason the message
  // says "signalée en rouge dans le formulaire" and means it.
  if (missing) input.closest("details")?.setAttribute("open", "");
}

/**
 * Flags every required input the vehicle table could not fill.
 *
 * An empty field is not an oversight to be papered over: the database has no
 * figure for it, and the simulation cannot run without one. Saying so at the
 * field is what turns "the button did nothing" into "this measurement is
 * missing".
 */
function flagMissing(): number {
  let missing = 0;
  for (const id of REQUIRED_INPUTS) {
    const input = byId<HTMLInputElement>(id);
    const empty = !input || Number.isNaN(input.valueAsNumber);
    if (empty) missing += 1;
    flag(id, empty);
  }
  return missing;
}

/**
 * Opens any disclosure holding a measurement that is still missing.
 *
 * The rule the fold has to obey: a required field that is empty is never
 * hidden. It is not only about starting from a blank form — several listed
 * vehicles have figures the table does not carry, and folding those away
 * would hide exactly the fields that stop the search from running.
 *
 * Opening is not flagging. A field is marked red when a search was asked for
 * and could not run; this only makes sure it can be seen.
 */
function openWhatIsMissing(): void {
  const missing = new Set<HTMLElement>();
  let count = 0;
  for (const id of REQUIRED_INPUTS) {
    const input = byId<HTMLInputElement>(id);
    if (!input || !Number.isNaN(input.valueAsNumber)) continue;
    count += 1;
    const holder = input.closest("details");
    if (holder) missing.add(holder);
  }
  for (const holder of missing) holder.setAttribute("open", "");

  const note = byId("vehicle-missing");
  if (note) {
    const { locale } = store.get().preferences;
    note.textContent =
      count === 0
        ? ""
        : locale === "en"
          ? ` — ${count} still to fill in`
          : ` — ${count} à renseigner`;
  }
}

function applyPreset(id: string): void {
  const preset = vehicleById(id);
  if (!preset) return;
  // A field the database does not know is **emptied**, not left alone.
  // Leaving it would keep whatever the previously selected vehicle put there:
  // pick the LBX then the EV3, and the Kia would show 2.029 m over its
  // mirrors — the Lexus figure, wearing the Kia's name. An empty field reads
  // as NaN, which the core rejects by name, so the driver is told which
  // measurement is missing instead of being given someone else's.
  const set = (field: string, value: number | null, digits = 3) => {
    const input = byId<HTMLInputElement>(field);
    if (input) input.value = value === null ? "" : value.toFixed(digits);
  };
  set("wheelbase", preset.wheelbase);
  set("length", preset.length);
  set("front-overhang", preset.front_overhang);
  set("body-width", preset.width);
  set("mirror-width", preset.mirror_width);
  set("mirror-width-folded", preset.mirror_width_folded);
  set("ground-clearance", preset.ground_clearance);
  set("radius", preset.min_turning_radius, 2);
  // Say where the number came from: it is not the one on the spec sheet, and
  // someone checking against the manufacturer would otherwise think it wrong.
  const note = byId("radius-note");
  if (note) {
    note.textContent =
      preset.min_turning_radius !== null && preset.published_radius !== null
        ? `— déduit de ${preset.published_radius.toFixed(1)} m entre trottoirs`
        : "— non publié pour ce modèle, à renseigner";
  }
  flagMissing();
  clearResult();
}

/**
 * The vehicle field: type to filter, arrows to choose, or just type a name.
 *
 * One control rather than a search box beside a dropdown. A name the table
 * does not know is not rejected — it is someone's own vehicle, and they fill
 * in its measurements themselves, which is why nothing is selected at the
 * start.
 *
 * The awkward decisions — where an arrow lands at the ends, what Enter
 * settles on — are in `ui/combobox.ts`, pure and tested. What is left here is
 * listeners and attributes.
 */
function fillPresets(): void {
  const field = byId<HTMLInputElement>("vehicle-name");
  const list = byId<HTMLUListElement>("vehicle-list");
  if (!field || !list) return;

  let matches: readonly VehiclePreset[] = [];
  let highlighted = NONE;

  const close = (): void => {
    list.hidden = true;
    field.setAttribute("aria-expanded", "false");
    field.removeAttribute("aria-activedescendant");
    highlighted = NONE;
  };

  const paint = (): void => {
    list.innerHTML = matches
      .map(
        (vehicle, i) =>
          `<li id="vehicle-option-${i}" role="option" data-index="${i}"
             aria-selected="${i === highlighted}"
             class="cursor-pointer px-2 py-1 ${
               i === highlighted ? "bg-accent text-ink" : "text-fg"
             }">${vehicle.label}</li>`,
      )
      .join("");
    if (highlighted !== NONE) {
      field.setAttribute("aria-activedescendant", `vehicle-option-${highlighted}`);
      // Keyboard navigation is useless if the highlight scrolls out of sight.
      list.children[highlighted]?.scrollIntoView({ block: "nearest" });
    } else {
      field.removeAttribute("aria-activedescendant");
    }
  };

  const open = (query: string): void => {
    matches = searchVehicles(query);
    highlighted = NONE;
    list.hidden = matches.length === 0;
    field.setAttribute("aria-expanded", String(matches.length > 0));
    paint();

    const note = byId("preset-note");
    if (note) {
      const { locale } = store.get().preferences;
      // Saying so beats an empty list, which reads as a broken page rather
      // than as a filter that matched nothing.
      note.textContent =
        query !== "" && matches.length === 0
          ? locale === "en"
            ? "No model matches — enter your own measurements below."
            : "Aucun modèle ne correspond — saisis tes propres mesures ci-dessous."
          : "";
    }
  };

  /** Applies a vehicle, or leaves the fields alone for an unknown name. */
  const settle = (): void => {
    const typed = field.value.trim().toLocaleLowerCase();
    const exact = VEHICLES.find((v) => v.label.toLocaleLowerCase() === typed);
    const chosen = commit(matches, highlighted, exact);
    close();
    if (!chosen) return;
    field.value = chosen.label;
    applyPreset(chosen.id);
    clearResult();
    draw();
  };

  field.addEventListener("input", () => {
    open(field.value);
  });
  field.addEventListener("focus", () => {
    open(field.value);
  });
  field.addEventListener("keydown", (event) => {
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp":
        event.preventDefault();
        if (list.hidden) open(field.value);
        highlighted = nextHighlight(
          highlighted,
          matches.length,
          event.key === "ArrowDown" ? 1 : -1,
        );
        paint();
        break;
      case "Enter":
        // Only when the list is open: otherwise Enter belongs to the form,
        // and swallowing it would break submitting from the keyboard.
        if (!list.hidden) {
          event.preventDefault();
          settle();
        }
        break;
      case "Escape":
        close();
        break;
      case "Tab":
        settle();
        break;
      default:
        break;
    }
  });

  list.addEventListener("mousedown", (event) => {
    // mousedown, not click: the field loses focus first, and a blur handler
    // would have closed the list before the click ever landed.
    const option = (event.target as HTMLElement).closest("li");
    if (!option) return;
    event.preventDefault();
    highlighted = Number(option.dataset["index"]);
    settle();
  });

  field.addEventListener("blur", () => {
    // Deferred, so a click on an option is still on its way.
    setTimeout(close, 120);
  });
  document.addEventListener("click", (event) => {
    if (event.target !== field && !list.contains(event.target as Node)) close();
  });

  for (const id of REQUIRED_INPUTS) {
    byId<HTMLInputElement>(id)?.addEventListener("input", () => {
      const input = byId<HTMLInputElement>(id);
      flag(id, !input || Number.isNaN(input.valueAsNumber));
    });
  }
}

function syncGateControls(): void {
  const swinging = byId<HTMLSelectElement>("gate-kind")?.value === "swinging";
  const box = byId("leaf-box");
  if (!box) return;
  box.classList.toggle("hidden", !swinging);
  box.classList.toggle("grid", swinging);
}

/** Bounds the opening angle by what the leaves can actually hold. */
async function syncMaxAngle(): Promise<void> {
  const label = byId("angle-max");
  const slider = byId<HTMLInputElement>("open-angle");
  if (!label || !slider) return;
  if (byId<HTMLSelectElement>("gate-kind")?.value !== "swinging") {
    label.textContent = "";
    store.set({ maxAngleDegrees: null });
    return;
  }
  try {
    const radians = await probe.maxGateAngle(readScene(store.get().preferences.units));
    const degrees = Math.round((radians * 180) / Math.PI);
    store.set({ maxAngleDegrees: degrees });
    slider.max = String(degrees);
    if (Number(slider.value) > degrees) {
      slider.value = String(degrees);
      const shown = byId("angle-value");
      if (shown) shown.textContent = slider.value;
      draw();
    }
    // Dialling an angle where the leaf passes through its own post would be
    // meaningless, so the control stops there.
    label.textContent = `maximum ${degrees}° avant que le vantail ne touche le pilier`;
  } catch {
    label.textContent = "";
    store.set({ maxAngleDegrees: null });
  }
}

/* ---------------------------------------------------------------- wiring */

fillPresets();
syncGateControls();

const form = byId("params");
form?.addEventListener("input", (event) => {
  const target = event.target;
  if (target instanceof HTMLInputElement && target.id === "open-angle") {
    const shown = byId("angle-value");
    if (shown) shown.textContent = target.value;
  }
  openWhatIsMissing();
  clearResult();
  draw();
});
form?.addEventListener("change", () => {
  syncGateControls();
  void syncMaxAngle();
  clearResult();
  draw();
});

/**
 * Adopts a new language or unit system.
 *
 * The result on screen was computed and written in the old one, so it is
 * cleared rather than relabelled: converting a finished verdict would be
 * fine, but converting the search that produced it would not, and a verdict
 * whose figures no longer match the form beside it is worse than none.
 */
function adopt(next: Preferences): void {
  const previous = store.get().preferences;
  if (previous.locale === next.locale && previous.units === next.units) return;

  convertFields(previous.units, next.units);
  store.set({ preferences: next });
  savePreferences(next, preferencesStorage);
  applyLanguage(next);
  clearResult();
  void syncMaxAngle();
  draw();
}

byId<HTMLSelectElement>("locale")?.addEventListener("change", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLSelectElement)) return;
  // Language alone. Changing it never moves the units: someone reading in
  // English has said nothing about the tape they measured their gate with.
  adopt({
    locale: target.value as Preferences["locale"],
    units: store.get().preferences.units,
  });
});

byId<HTMLSelectElement>("units")?.addEventListener("change", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLSelectElement)) return;
  adopt({
    locale: store.get().preferences.locale,
    units: target.value as Preferences["units"],
  });
});

byId("play")?.addEventListener("click", () => {
  if (store.get().playing) stopPlaying();
  else startPlaying();
});

// The scrubber redraws on the display's rhythm rather than on every input
// event: a fast sweep otherwise queues far more redraws than the screen shows.
let pending = false;
byId<HTMLInputElement>("scrub")?.addEventListener("input", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLInputElement)) return;
  // Taking hold of the scrubber takes over from the clock. Leaving both
  // running would have them fight for the same value every frame.
  if (store.get().playing) stopPlaying();
  const fraction = Number(target.value) / 100;
  if (pending) return;
  pending = true;
  requestAnimationFrame(() => {
    pending = false;
    store.set({ position: fraction });
  });
});

form?.addEventListener("submit", async (event) => {
  event.preventDefault();
  // While a search runs the button stops it, rather than being inert or
  // silently queueing another one.
  if (store.get().busy) {
    client.cancel();
    store.set({
      busy: false,
      message: text(store.get().preferences.locale, "msg.interrupted"),
      progress: "",
    });
    return;
  }
  // The slider is bounded by max_gate_angle, but a bound that fails silently
  // would let someone compute a scene where the leaf passes through its own
  // post — an answer that means nothing. Refuse rather than pretend.
  const requested = readScene(store.get().preferences.units);
  const { maxAngleDegrees } = store.get();
  if (requested.gate.kind === "swinging" && maxAngleDegrees !== null) {
    const degrees = (requested.gate.open_angle * 180) / Math.PI;
    if (degrees > maxAngleDegrees + 0.5) {
      store.set({
        message: leafTooOpen(degrees, maxAngleDegrees, store.get().preferences.locale),
      });
      return;
    }
  }

  // Nothing to compute while a measurement is missing. Running anyway would
  // send a NaN across the boundary and come back with one rejected field,
  // naming only the first of them.
  const missing = flagMissing();
  if (missing > 0) {
    store.set({
      message: missingMeasurements(missing, store.get().preferences.locale),
      alternatives: [],
    });
    return;
  }

  // Some answers need no search. A vehicle at least as wide as its opening
  // cannot pass, and that is *proved* — `(W − w) / 2` is a ceiling no
  // trajectory beats — where a search would only fail to find something. So
  // refuse here, and refuse on the mirror width this run would actually use:
  // folding the mirrors is precisely what gets a car through a gate it does
  // not otherwise fit.
  const refusal = refuseOnWidth(
    requested.right_post.inner_edge_x - requested.left_post.inner_edge_x,
    readRequest(store.get().preferences.units).vehicle.mirror_width,
  );
  if (refusal) {
    store.set({ outcome: refusal, message: "", alternatives: [], busy: false });
    return;
  }

  // The exhaustive sweep runs first and reports nothing — it has no nodes to
  // count — so this is what the interface shows until the first progress
  // message says the planner has taken over.
  store.set({
    busy: true,
    outcome: null,
    message: "",
    progress: text(store.get().preferences.locale, "msg.firstPass"),
    alternatives: [],
  });

  try {
    const response = await client.solve(
      readRequest(store.get().preferences.units),
      (maxMoves, expanded, budget) => {
        store.set({
          progress: searchProgress(
            maxMoves,
            expanded,
            budget,
            store.get().preferences.locale,
          ),
        });
      },
    );
    // One path for all three outcomes. `verdictOf` is what knows that an
    // exhausted budget proves nothing, so no caller has to remember it.
    store.set({
      alternatives: response.alternatives,
      selected: 0,
      position: 1,
      outcome: verdictOf(response),
      message: "",
    });
  } catch (thrown) {
    const error = thrown as ErrorDto;
    if (error.code !== CANCELLED) {
      // Point at the field the boundary named, so the sentence and the form
      // say the same thing.
      const input = error.field ? FIELD_INPUTS[error.field] : undefined;
      if (input) flag(input, true);
      store.set({ message: errorMessage(error, store.get().preferences.locale) });
    }
  } finally {
    // Only clear the flag if nothing took over in the meantime.
    store.set({ busy: client.busy, progress: "" });
  }
});

byId("run-min-road")?.addEventListener("click", async () => {
  const absent = flagMissing();
  if (absent > 0) {
    store.set({
      message: missingMeasurements(absent, store.get().preferences.locale),
    });
    return;
  }
  store.set({
    busy: true,
    message: text(store.get().preferences.locale, "msg.minRoadSearching"),
    outcome: null,
  });
  try {
    const width = await client.minRoad(readRequest(store.get().preferences.units));
    store.set({ message: minRoadResult(width, store.get().preferences) });
  } catch (thrown) {
    const error = thrown as ErrorDto;
    if (error.code !== CANCELLED) {
      const input = error.field ? FIELD_INPUTS[error.field] : undefined;
      if (input) flag(input, true);
      store.set({ message: errorMessage(error, store.get().preferences.locale) });
    }
  } finally {
    store.set({ busy: client.busy });
  }
});

// The page ships in French with metric fields; the first thing to do is put
// it into whatever was chosen last time, or guessed from the browser.
{
  const initial = store.get().preferences;
  const locale = byId<HTMLSelectElement>("locale");
  if (locale) locale.value = initial.locale;
  const units = byId<HTMLSelectElement>("units");
  if (units) units.value = initial.units;
  convertFields("metric", initial.units);
  applyLanguage(initial);
}

draw();
