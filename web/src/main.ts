import {
  centimetres,
  confidenceLabel,
  errorMessage,
  metres,
  moves,
} from "./domain/labels";
import type { ErrorDto, ManeuverDto, SceneDto, VehicleDto } from "./domain/types";
import { VEHICLES, searchVehicles, vehicleById } from "./domain/vehicles";
import { BANDS, pathToPrimitives } from "./render/path";
import { projectionFor } from "./render/projection";
import { boundsFor, sceneToPrimitives } from "./render/scene";
import { renderSvg } from "./render/svg";
import { createStore } from "./state/store";
import { arrivesFromTheRight, readRequest, readScene } from "./ui/form";
import { CANCELLED, SolverClient } from "./worker/client";

const VIEWPORT = { width: 1000, height: 600 };
const client = new SolverClient();
// Instant queries get their own worker. They must neither cancel a search in
// flight nor be cancelled by one — which is exactly what happened when
// clearResult() started cancelling: it killed the angle query fired a line
// earlier, and the slider was never bounded.
const probe = new SolverClient();

const store = createStore({
  busy: false,
  verdict: "",
  progress: "",
  alternatives: [] as ManeuverDto[],
  selected: 0,
  position: 1,
  maxAngleDegrees: null as number | null,
});

const byId = <T extends HTMLElement>(id: string): T | null =>
  document.getElementById(id) as T | null;

/* ------------------------------------------------------------------ draw */

/** Redraws the plan. Cheap enough to run on every change. */
function draw(): void {
  const svg = document.getElementById("plan");
  if (!(svg instanceof SVGSVGElement)) return;

  let scene: SceneDto;
  let vehicle: VehicleDto;
  try {
    scene = readScene();
    vehicle = readRequest().vehicle;
  } catch {
    return;
  }

  const projection = projectionFor(boundsFor(scene), VIEWPORT, arrivesFromTheRight());
  const primitives = [...sceneToPrimitives(scene)];

  const { alternatives, selected, position } = store.get();
  const current = alternatives[selected];
  if (current) {
    primitives.push(...pathToPrimitives(current, vehicle, position));
  }
  renderSvg(primitives, svg, projection);
}

/* --------------------------------------------------------------- reports */

const BAND_NAMES = ["au large", "vigilance", "proche", "très proche"] as const;
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
  legend.innerHTML = `${BAND_NAMES.map((name, i) => {
    const range =
      i === 0
        ? `plus de ${BANDS[0] * 100} cm`
        : i === 3
          ? `moins de ${BANDS[2] * 100} cm`
          : `${BANDS[i]! * 100} à ${BANDS[i - 1]! * 100} cm`;
    return `<span class="flex items-center gap-1"><i class="inline-block h-2 w-4 rounded" style="background:var(${BAND_TOKENS[i]})"></i>${name} (${range})</span>`;
  }).join("")}<span class="flex items-center gap-1"><i class="inline-block h-2 w-4 rounded" style="background:var(--color-overhang)"></i>surplomb du trottoir</span><span class="ml-auto">trait plein : marche avant · pointillé : marche arrière</span>`;
}

function renderStats(maneuver: ManeuverDto): void {
  const stats = byId("stats");
  if (!stats) return;
  const card = (key: string, value: string) =>
    `<div class="rounded border border-stone-200 bg-white px-3 py-2">
       <dt class="text-xs uppercase tracking-wide text-stone-500">${key}</dt>
       <dd class="mt-0.5 text-lg">${value}</dd>
     </div>`;

  // Two clearances, because they answer different questions: the gateway is
  // what the driver asked about, the overall figure may be a kerb metres away.
  stats.innerHTML = [
    card("Manœuvres", String(maneuver.moves)),
    card("Marge dans le passage", centimetres(maneuver.min_clearance_in_gateway)),
    card("Marge minimale du trajet", centimetres(maneuver.min_clearance)),
    card("Distance parcourue", metres(maneuver.distance)),
    card("Sous 25 cm", metres(maneuver.metres_under_25cm)),
    card("Sous 10 cm", metres(maneuver.metres_under_10cm)),
    // Shown only when it has something to say: a zero on every ordinary
    // trajectory would teach nobody anything.
    ...(maneuver.metres_overhanging > 0
      ? [card("Surplomb du trottoir", metres(maneuver.metres_overhanging))]
      : []),
  ].join("");
}

function renderAlternatives(): void {
  const box = byId("alternatives");
  if (!box) return;
  const { alternatives, selected } = store.get();
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
               ? "border-stone-900 bg-stone-900 text-stone-50"
               : "border-stone-300 bg-white"
           }">
           <span class="block text-sm font-medium">${moves(a.moves)}</span>
           <span class="block text-xs opacity-80">${centimetres(
             a.min_clearance_in_gateway,
           )} · ${confidenceLabel(a.confidence)}</span>
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

store.subscribe(() => {
  const { verdict, alternatives, selected, busy, progress } = store.get();
  const output = byId("verdict");
  if (output) output.textContent = verdict;

  const bar = byId("progress");
  if (bar) bar.classList.toggle("hidden", !busy);
  const note = byId("progress-note");
  if (note) note.textContent = progress;

  const run = byId("run");
  if (run) {
    run.textContent = busy ? "Arrêter le calcul" : "Rechercher l'entrée";
  }

  renderAlternatives();
  const current = alternatives[selected];
  const scrub = byId<HTMLInputElement>("scrub");
  if (scrub) scrub.disabled = !current;
  renderLegend(Boolean(current));
  if (current) renderStats(current);
  else byId("stats")?.replaceChildren();

  draw();
});

/* ------------------------------------------------------------------ form */

function clearResult(): void {
  // Whatever is running answers a question that has just changed, so it is
  // abandoned rather than left to overwrite the screen with a stale verdict.
  client.cancel();
  store.set({
    alternatives: [],
    selected: 0,
    position: 1,
    verdict: "",
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
  "pavement",
  "kerb",
  "road",
  "kerb-height",
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

/** Marks or clears one input as needing attention. */
function flag(id: string, missing: boolean): void {
  const input = byId<HTMLInputElement>(id);
  if (!input) return;
  input.classList.toggle("border-red-500", missing);
  input.classList.toggle("bg-red-50", missing);
  input.setAttribute("aria-invalid", missing ? "true" : "false");
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

function fillPresets(): void {
  const select = byId<HTMLSelectElement>("preset");
  if (!select) return;

  const render = (query: string) => {
    const matches = searchVehicles(query);
    select.innerHTML = matches
      .map((v) => `<option value="${v.id}">${v.label}</option>`)
      .join("");
    select.disabled = matches.length === 0;
    const note = byId("preset-note");
    if (note) {
      // Saying so beats an empty dropdown, which reads as a broken page
      // rather than as a filter that matched nothing.
      note.textContent =
        matches.length === 0
          ? "Aucun modèle ne correspond."
          : matches.length < VEHICLES.length
            ? `${matches.length} sur ${VEHICLES.length} modèles`
            : "";
    }
    const first = matches[0];
    if (first) applyPreset(first.id);
  };

  render("");
  select.addEventListener("change", () => {
    applyPreset(select.value);
  });
  for (const id of REQUIRED_INPUTS) {
    byId<HTMLInputElement>(id)?.addEventListener("input", () => {
      const input = byId<HTMLInputElement>(id);
      flag(id, !input || Number.isNaN(input.valueAsNumber));
    });
  }
  byId<HTMLInputElement>("preset-search")?.addEventListener("input", (event) => {
    render((event.target as HTMLInputElement).value);
  });
}

/** Shows the leaf controls only when they apply. */
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
    const radians = await probe.maxGateAngle(readScene());
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
  clearResult();
  draw();
});
form?.addEventListener("change", () => {
  syncGateControls();
  void syncMaxAngle();
  clearResult();
  draw();
});

// The scrubber redraws on the display's rhythm rather than on every input
// event: a fast sweep otherwise queues far more redraws than the screen shows.
let pending = false;
byId<HTMLInputElement>("scrub")?.addEventListener("input", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLInputElement)) return;
  const fraction = Number(target.value) / 100;
  const label = byId("scrub-label");
  if (label) label.textContent = `${target.value} %`;
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
    store.set({ busy: false, verdict: "Calcul interrompu.", progress: "" });
    return;
  }
  // The slider is bounded by max_gate_angle, but a bound that fails silently
  // would let someone compute a scene where the leaf passes through its own
  // post — an answer that means nothing. Refuse rather than pretend.
  const requested = readScene();
  const { maxAngleDegrees } = store.get();
  if (requested.gate.kind === "swinging" && maxAngleDegrees !== null) {
    const degrees = (requested.gate.open_angle * 180) / Math.PI;
    if (degrees > maxAngleDegrees + 0.5) {
      store.set({
        verdict:
          `Un vantail ne peut pas s'ouvrir à ${degrees.toFixed(0)}° avec cet axe : ` +
          `il traverserait le pilier. Le maximum est ${maxAngleDegrees}°.`,
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
      verdict:
        missing === 1
          ? "Une mesure manque, signalée en rouge dans le formulaire."
          : `${missing} mesures manquent, signalées en rouge dans le formulaire.`,
      alternatives: [],
    });
    return;
  }

  // The exhaustive sweep runs first and reports nothing — it has no nodes to
  // count — so this is what the interface shows until the first progress
  // message says the planner has taken over.
  store.set({
    busy: true,
    verdict: "Calcul en cours…",
    progress: "Calcul des trajectoires en une manœuvre…",
    alternatives: [],
  });

  try {
    const response = await client.solve(
      readRequest(),
      (moves, expanded, budget) => {
        // "Situations" rather than nodes: what the planner counts is the
        // vehicle placed somewhere, facing some way, in some gear. And the
        // ceiling is named, because a running count without its scale says
        // nothing about where this ends.
        const count = expanded.toLocaleString("fr-FR");
        const ceiling = budget.toLocaleString("fr-FR");
        store.set({
          progress: `Calcul des trajectoires en ${moves} manœuvres — ${count} situations essayées sur ${ceiling} au plus`,
        });
      },
    );
    if (response.alternatives.length === 0) {
      // An exhaustive sweep proves absence; a heuristic one does not.
      store.set({
        verdict: response.budget_exhausted
          ? "Aucune entrée trouvée dans le budget imparti. La recherche est heuristique : cela ne prouve pas que l'entrée soit impossible."
          : "Aucune entrée n'est possible avec ces mesures.",
      });
      return;
    }

    const best = response.alternatives[0]!;
    const elsewhere =
      best.min_clearance < best.min_clearance_in_gateway - 1e-9
        ? ` Ailleurs sur le trajet, la marge descend à ${centimetres(best.min_clearance)} — sur la voirie, pas dans le passage.`
        : "";

    store.set({
      alternatives: response.alternatives,
      selected: 0,
      position: 1,
      verdict: `Entrée possible en ${moves(best.moves)}, avec ${centimetres(
        best.min_clearance_in_gateway,
      )} de marge dans le passage (${confidenceLabel(best.confidence)}).${elsewhere}`,
    });
  } catch (thrown) {
    const error = thrown as ErrorDto;
    if (error.code !== CANCELLED) {
      // Point at the field the boundary named, so the sentence and the form
      // say the same thing.
      const input = error.field ? FIELD_INPUTS[error.field] : undefined;
      if (input) flag(input, true);
      store.set({ verdict: errorMessage(error) });
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
      verdict:
        absent === 1
          ? "Une mesure manque, signalée en rouge dans le formulaire."
          : `${absent} mesures manquent, signalées en rouge dans le formulaire.`,
    });
    return;
  }
  store.set({ busy: true, verdict: "Recherche de la chaussée minimale…" });
  try {
    const width = await client.minRoad(readRequest());
    store.set({
      verdict:
        width === null
          ? "Aucune largeur de chaussée ne permet l'entrée en un mouvement : le passage lui-même est bloquant."
          : `Il faut au minimum ${metres(width)} de chaussée pour entrer en un seul mouvement.`,
    });
  } catch (thrown) {
    const error = thrown as ErrorDto;
    if (error.code !== CANCELLED) {
      const input = error.field ? FIELD_INPUTS[error.field] : undefined;
      if (input) flag(input, true);
      store.set({ verdict: errorMessage(error) });
    }
  } finally {
    store.set({ busy: client.busy });
  }
});

draw();
