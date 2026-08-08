import {
  centimetres,
  confidenceLabel,
  errorMessage,
  metres,
  moves,
} from "./domain/labels";
import type {
  ErrorDto,
  ManeuverDto,
  SceneDto,
  SolveRequest,
  VehicleDto,
} from "./domain/types";
import { pathToPrimitives } from "./render/path";
import { boundsFor, sceneToPrimitives } from "./render/scene";
import { projectionFor } from "./render/projection";
import { renderSvg } from "./render/svg";
import { createStore } from "./state/store";
import { SolverClient } from "./worker/client";

const VIEWPORT = { width: 1000, height: 600 };

/** Redraws the plan from scratch. Cheap enough to do on every change. */
function draw(
  scene: SceneDto,
  vehicle?: VehicleDto,
  maneuver?: ManeuverDto,
  position = 1,
): void {
  const svg = document.getElementById("plan");
  if (!(svg instanceof SVGSVGElement)) return;
  const projection = projectionFor(boundsFor(scene), VIEWPORT, false);
  const primitives = [...sceneToPrimitives(scene)];
  if (maneuver && vehicle) {
    primitives.push(...pathToPrimitives(maneuver, vehicle, position));
  }
  renderSvg(primitives, svg, projection);
}

const client = new SolverClient();
const store = createStore({ busy: false, verdict: "" });

const field = (id: string): HTMLInputElement => {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`missing input: ${id}`);
  }
  return element;
};

/** Reads the form into a request. Only a handful of controls for now. */
function readRequest(): SolveRequest {
  const opening = field("opening").valueAsNumber;
  const mirrors = field("mirrors").valueAsNumber;
  return {
    scene: {
      left_post: { inner_edge_x: -opening / 2, width: 0.55, depth: 0.55 },
      right_post: { inner_edge_x: opening / 2, width: 0.55, depth: 0.55 },
      wall_thickness: 0.3,
      pavement_width: 1.2,
      dropped_kerb_width: opening + 0.8,
      road_width: field("road").valueAsNumber,
      gate: { kind: "sliding" },
    },
    vehicle: {
      wheelbase: 2.58,
      length: 4.19,
      front_overhang: 0.85,
      width: 1.825,
      mirror_width: mirrors,
      min_turning_radius: field("radius").valueAsNumber,
    },
    forward_only: null,
  };
}

const verdict = document.getElementById("verdict");
store.subscribe(() => {
  if (verdict) verdict.textContent = store.get().verdict;
});

const form = document.getElementById("params");
form?.addEventListener("input", () => {
  draw(readRequest().scene);
});
draw(readRequest().scene);

form?.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (store.get().busy) return;
  store.set({ busy: true, verdict: "Calcul en cours…" });

  try {
    const response = await client.solve(readRequest());
    const best = response.alternatives[0];

    if (!best) {
      // An exhaustive sweep proves absence; a heuristic one does not. Saying
      // otherwise would overstate what the search actually established.
      store.set({
        verdict: response.budget_exhausted
          ? "Aucune entrée trouvée dans le budget imparti. La recherche est heuristique : cela ne prouve pas que l'entrée soit impossible."
          : "Aucune entrée n'est possible avec ces mesures.",
      });
      return;
    }

    // Two clearances, because they answer different questions: the gateway
    // one is what the driver asked about, the overall one may be a kerb
    // several metres away.
    const gateway = centimetres(best.min_clearance_in_gateway);
    const overall = centimetres(best.min_clearance);
    const elsewhere =
      best.min_clearance < best.min_clearance_in_gateway - 1e-9
        ? ` Ailleurs sur le trajet, la marge descend à ${overall}.`
        : "";

    const request = readRequest();
    draw(request.scene, request.vehicle, best);

    store.set({
      verdict:
        `Entrée possible en ${moves(best.moves)}. ` +
        `Marge dans le passage : ${gateway} (${confidenceLabel(best.confidence)}).` +
        elsewhere +
        ` Trajet de ${metres(best.distance)}, dont ${metres(best.metres_under_25cm)} sous 25 cm.`,
    });
  } catch (thrown) {
    store.set({ verdict: errorMessage(thrown as ErrorDto) });
  } finally {
    store.set({ busy: false });
  }
});
