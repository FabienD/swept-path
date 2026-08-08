import {
  centimetres,
  confidenceLabel,
  errorMessage,
  metres,
  moves,
} from "./domain/labels";
import type { ErrorDto, SolveRequest } from "./domain/types";
import { createStore } from "./state/store";
import { SolverClient } from "./worker/client";

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

document.getElementById("params")?.addEventListener("submit", async (event) => {
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
